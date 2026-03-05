//! Voice command system for LLM-powered voice control
//!
//! Triggered by holding both gamepad triggers (L2+R2). Voice is transcribed
//! in real-time using Soniox and sent to an LLM which interprets the command
//! and returns a menu of potential actions.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;

use bevy::prelude::*;
use crossbeam_channel::{unbounded, Receiver, Sender};
use serde::{Deserialize, Serialize};
use tungstenite::{connect, Message};
use tungstenite::stream::MaybeTlsStream;

use crate::commands::Command;

// ============================================================================
// Configuration
// ============================================================================

/// Available LLM models with their metadata (fetched from Requesty API)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LlmModelInfo {
    pub id: String,
    #[serde(default)]
    pub owned_by: String,
    #[serde(default)]
    pub created: i64,
}

/// Voice command configuration
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VoiceCommandConfig {
    /// Selected LLM model ID (e.g., "anthropic/claude-3-haiku")
    pub model: String,
    /// Speech-to-text provider (placeholder for future use)
    pub stt_provider: String,
    /// Requesty.ai API key (optional - falls back to file if not set)
    #[serde(default)]
    pub requesty_api_key: String,
    /// Soniox API key (optional - falls back to file if not set)
    #[serde(default)]
    pub soniox_api_key: String,
}

impl Default for VoiceCommandConfig {
    fn default() -> Self {
        Self {
            model: "anthropic/claude-3-haiku".to_string(),
            stt_provider: "soniox".to_string(),
            requesty_api_key: String::new(),
            soniox_api_key: String::new(),
        }
    }
}

/// State for fetching models from Requesty API
#[derive(Clone, Debug, Default)]
pub struct ModelFetchState {
    pub models: Vec<LlmModelInfo>,
    pub loading: bool,
    pub error: Option<String>,
    pub filter: String,
}

/// Fetch available models from Requesty API
pub fn fetch_models_from_requesty(
    result_tx: Sender<Result<Vec<LlmModelInfo>, String>>,
) {
    let api_key = match load_api_key() {
        Some(key) => key,
        None => {
            let _ = result_tx.send(Err("No Requesty API key found".to_string()));
            return;
        }
    };

    thread::spawn(move || {
        let client = reqwest::blocking::Client::new();
        let response = client
            .get("https://router.requesty.ai/v1/models")
            .header("Authorization", format!("Bearer {}", api_key))
            .send();

        match response {
            Ok(resp) => {
                if resp.status().is_success() {
                    match resp.json::<ModelsResponse>() {
                        Ok(models_resp) => {
                            let _ = result_tx.send(Ok(models_resp.data));
                        }
                        Err(e) => {
                            let _ = result_tx.send(Err(format!("Failed to parse models: {}", e)));
                        }
                    }
                } else {
                    let error_text = resp.text().unwrap_or_default();
                    let _ = result_tx.send(Err(format!("API error: {}", error_text)));
                }
            }
            Err(e) => {
                let _ = result_tx.send(Err(format!("Request failed: {}", e)));
            }
        }
    });
}

#[derive(Debug, Deserialize)]
struct ModelsResponse {
    data: Vec<LlmModelInfo>,
}

impl VoiceCommandConfig {
    /// Load config from disk
    pub fn load() -> Self {
        Self::config_path()
            .and_then(|path| fs::read_to_string(&path).ok())
            .and_then(|data| serde_json::from_str(&data).ok())
            .unwrap_or_default()
    }

    /// Save config to disk
    pub fn save(&self) -> Result<(), String> {
        let path = Self::config_path().ok_or("No config directory")?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let data = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        fs::write(&path, data).map_err(|e| e.to_string())
    }

    fn config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|d| d.join("gradesta").join("voice_command.json"))
    }
}

/// Bevy resource holding the voice command config
#[derive(Resource)]
pub struct VoiceCommandConfigRes(pub VoiceCommandConfig);

/// Channel for receiving model fetch results
#[derive(Resource)]
pub struct ModelFetchChannel {
    pub tx: Sender<Result<Vec<LlmModelInfo>, String>>,
    pub rx: Receiver<Result<Vec<LlmModelInfo>, String>>,
}

impl Default for ModelFetchChannel {
    fn default() -> Self {
        let (tx, rx) = unbounded();
        Self { tx, rx }
    }
}

// ============================================================================
// State Types
// ============================================================================

/// Voice command processing state
#[derive(Clone, Debug)]
pub enum VoiceCommandState {
    /// Recording voice input with real-time transcription
    Recording {
        samples: Arc<Mutex<Vec<f32>>>,
        stop_signal: Arc<Mutex<bool>>,
        /// Live transcript updated in real-time as user speaks
        live_transcript: Arc<Mutex<String>>,
        /// WebSocket sender for streaming audio (held by recording thread)
        ws_audio_tx: Option<Sender<Vec<u8>>>,
        /// Current audio input level (0.0 - 1.0) for volume meter
        audio_level: Arc<Mutex<f32>>,
    },
    /// Finalizing transcription (brief state while WebSocket closes)
    Transcribing,
    /// LLM processing (initial query or after permission granted)
    Interpreting {
        transcript: String,
        cell_context: Option<CellContext>,
    },
    /// LLM requested to view cells - awaiting user permission
    AwaitingPermission {
        transcript: String,
        requested_targets: Vec<String>, // ["current", "north", "east"]
        reason: String,
        /// Selected button: 0 = Allow, 1 = Deny
        selected: usize,
    },
    /// Final menu shown, user selecting
    Selecting {
        transcript: String,
        interpretations: Vec<AgentInterpretation>,
        selected: usize,
    },
}

impl PartialEq for VoiceCommandState {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (VoiceCommandState::Recording { .. }, VoiceCommandState::Recording { .. }) => true,
            (VoiceCommandState::Transcribing, VoiceCommandState::Transcribing) => true,
            (
                VoiceCommandState::Interpreting { transcript: t1, .. },
                VoiceCommandState::Interpreting { transcript: t2, .. },
            ) => t1 == t2,
            (
                VoiceCommandState::AwaitingPermission { transcript: t1, selected: s1, .. },
                VoiceCommandState::AwaitingPermission { transcript: t2, selected: s2, .. },
            ) => t1 == t2 && s1 == s2,
            (
                VoiceCommandState::Selecting { transcript: t1, selected: s1, .. },
                VoiceCommandState::Selecting { transcript: t2, selected: s2, .. },
            ) => t1 == t2 && s1 == s2,
            _ => false,
        }
    }
}

impl Eq for VoiceCommandState {}

/// Cell content context provided to the LLM (after user grants permission)
#[derive(Clone, Debug, Default)]
pub struct CellContext {
    /// Content of current cell (if permitted)
    pub current: Option<String>,
    /// Direction -> content mapping (e.g., "north" -> "Hello world")
    pub directions: HashMap<String, String>,
}

/// An action the agent can suggest
#[derive(Clone, Debug)]
pub enum AgentAction {
    /// Execute a command script (semicolon-separated commands)
    /// e.g. "global.focus_url;insert_text \"ws://localhost:8080\";global.refresh"
    Script(String),
    /// Cancel the voice command (user-selectable option)
    Cancel,
}

/// A single interpretation from the LLM
#[derive(Clone, Debug)]
pub struct AgentInterpretation {
    pub action: AgentAction,
    pub confidence: f32,
    pub explanation: String,
}

// ============================================================================
// Event Channel
// ============================================================================

/// Events from voice command async operations
#[derive(Debug)]
pub enum VoiceCommandEvent {
    /// Real-time transcript update (words as they're spoken)
    TranscriptUpdate { text: String, is_final: bool },
    /// Transcription completed
    TranscriptionComplete { transcript: String },
    /// Transcription failed
    TranscriptionFailed { error: String },
    /// LLM response received
    LlmResponse { interpretations: Vec<AgentInterpretation> },
    /// LLM requested to view cells
    LlmRequestsView { targets: Vec<String>, reason: String },
    /// LLM request failed
    LlmFailed { error: String },
}

/// Channel for voice command async events
#[derive(Resource)]
pub struct VoiceCommandChannel {
    pub tx: Sender<VoiceCommandEvent>,
    pub rx: Receiver<VoiceCommandEvent>,
}

impl Default for VoiceCommandChannel {
    fn default() -> Self {
        let (tx, rx) = unbounded();
        Self { tx, rx }
    }
}

// ============================================================================
// API Configuration
// ============================================================================

/// Load the API key for LLM requests (Requesty.ai)
/// Checks config first, falls back to file
pub fn load_api_key() -> Option<String> {
    // Check config first
    let config = VoiceCommandConfig::load();
    if !config.requesty_api_key.is_empty() {
        return Some(config.requesty_api_key);
    }
    // Fall back to file
    let home = dirs::home_dir()?;
    let key_path = home.join(".config/gradesta/elves/simple-llm/requesty.ai/secret.key");
    fs::read_to_string(&key_path).ok().map(|s| s.trim().to_string())
}

/// Load the Soniox API key for speech-to-text
/// Checks config first, falls back to file
pub fn load_soniox_api_key() -> Option<String> {
    // Check config first
    let config = VoiceCommandConfig::load();
    if !config.soniox_api_key.is_empty() {
        return Some(config.soniox_api_key);
    }
    // Fall back to file
    let home = dirs::home_dir()?;
    let key_path = home.join(".config/gradesta/elves/simple-llm/soniox.com/secret.key");
    fs::read_to_string(&key_path).ok().map(|s| s.trim().to_string())
}

// ============================================================================
// Real-time Speech-to-Text (Soniox WebSocket)
// ============================================================================

/// Soniox WebSocket response for transcription tokens
#[derive(Debug, Deserialize)]
struct SonioxResponse {
    tokens: Option<Vec<SonioxToken>>,
    finished: Option<bool>,
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SonioxToken {
    text: String,
    #[serde(default)]
    is_final: bool,
}

/// Start real-time transcription via Soniox WebSocket
/// Returns a channel to send audio chunks to, and spawns a thread to handle WebSocket communication
pub fn start_realtime_transcription(
    live_transcript: Arc<Mutex<String>>,
    result_tx: Sender<VoiceCommandEvent>,
) -> Option<Sender<Vec<u8>>> {
    let api_key = match load_soniox_api_key() {
        Some(key) => {
            eprintln!("Soniox API key loaded ({} chars)", key.len());
            key
        }
        None => {
            eprintln!("No Soniox API key found at ~/.config/gradesta/elves/simple-llm/soniox.com/secret.key");
            let _ = result_tx.send(VoiceCommandEvent::TranscriptionFailed {
                error: "No Soniox API key found".to_string(),
            });
            return None;
        }
    };

    // Channel for sending audio chunks to the WebSocket thread
    let (audio_tx, audio_rx): (Sender<Vec<u8>>, Receiver<Vec<u8>>) = unbounded();

    thread::spawn(move || {
        if let Err(e) = run_soniox_websocket(api_key, audio_rx, live_transcript.clone(), result_tx.clone()) {
            eprintln!("Soniox WebSocket error: {}", e);
            let _ = result_tx.send(VoiceCommandEvent::TranscriptionFailed {
                error: format!("Soniox transcription failed: {}", e),
            });
        }
    });

    Some(audio_tx)
}

fn run_soniox_websocket(
    api_key: String,
    audio_rx: Receiver<Vec<u8>>,
    live_transcript: Arc<Mutex<String>>,
    result_tx: Sender<VoiceCommandEvent>,
) -> Result<(), String> {
    // Connect to Soniox WebSocket (native-tls feature handles wss:// automatically)
    let url = "wss://stt-rt.soniox.com/transcribe-websocket";
    eprintln!("Connecting to Soniox WebSocket: {}", url);

    let (mut ws, _response) = connect(url)
        .map_err(|e| {
            // Try to extract more details from HTTP errors
            match &e {
                tungstenite::Error::Http(response) => {
                    let status = response.status();
                    let body = response.body().as_ref()
                        .map(|b| String::from_utf8_lossy(b).to_string())
                        .unwrap_or_else(|| "(no body)".to_string());
                    eprintln!("Soniox HTTP error: {} - Body: {}", status, body);
                    format!("HTTP error: {} - {}", status, body)
                }
                _ => {
                    eprintln!("Soniox connection error: {:?}", e);
                    format!("WebSocket connection failed: {}", e)
                }
            }
        })?;

    eprintln!("Soniox WebSocket connected, sending config...");

    // Send configuration message with all required fields for raw PCM
    let config = serde_json::json!({
        "api_key": api_key,
        "model": "stt-rt-preview",
        "audio_format": "s16le",
        "sample_rate": 16000,
        "num_channels": 1,
        "language_hints": ["en"]
    });

    eprintln!("Sending Soniox config: {}", config);
    ws.send(Message::Text(config.to_string()))
        .map_err(|e| format!("Failed to send config: {}", e))?;

    // Set non-blocking mode for reading
    match ws.get_ref() {
        MaybeTlsStream::Plain(stream) => {
            stream.set_nonblocking(true).ok();
        }
        MaybeTlsStream::NativeTls(tls_stream) => {
            tls_stream.get_ref().set_nonblocking(true).ok();
        }
        _ => {
            eprintln!("Warning: Could not set non-blocking mode for WebSocket stream");
        }
    }

    let mut final_transcript = String::new();
    let mut interim_transcript = String::new();

    let mut audio_chunks_sent = 0;
    let mut total_bytes_sent = 0;
    let mut end_of_stream_sent = false;
    let mut end_of_stream_time: Option<std::time::Instant> = None;

    loop {
        // Timeout: if we've sent end-of-stream and been waiting > 500ms, consider complete
        // Soniox doesn't reliably send "finished: true", so we use a short timeout
        if let Some(eos_time) = end_of_stream_time {
            if eos_time.elapsed() > std::time::Duration::from_millis(500) {
                eprintln!("Transcription complete (timeout after end-of-stream)");
                let final_text = format!("{}{}", final_transcript, interim_transcript);
                let _ = result_tx.send(VoiceCommandEvent::TranscriptionComplete {
                    transcript: final_text,
                });
                return Ok(());
            }
        }
        // Try to receive audio chunk (non-blocking) - only if we haven't sent end-of-stream
        if !end_of_stream_sent {
            match audio_rx.try_recv() {
                Ok(chunk) => {
                    let chunk_size = chunk.len();
                    // Send audio as binary frame
                    if let Err(e) = ws.send(Message::Binary(chunk)) {
                        eprintln!("Failed to send audio: {}", e);
                        break;
                    }
                    audio_chunks_sent += 1;
                    total_bytes_sent += chunk_size;
                    if audio_chunks_sent % 10 == 1 {
                        eprintln!("Sent {} audio chunks ({} bytes total)", audio_chunks_sent, total_bytes_sent);
                    }
                }
                Err(crossbeam_channel::TryRecvError::Empty) => {
                    // No audio available, continue to check for responses
                }
                Err(crossbeam_channel::TryRecvError::Disconnected) => {
                    // Recording stopped - send empty frame to signal end of audio
                    // Per Soniox docs: send empty frame, then wait for "finished" response
                    eprintln!("Audio channel closed after {} chunks ({} bytes), sending end-of-stream", audio_chunks_sent, total_bytes_sent);
                    if let Err(e) = ws.send(Message::Binary(vec![])) {
                        eprintln!("Failed to send end-of-stream: {}", e);
                    }
                    end_of_stream_sent = true;
                    end_of_stream_time = Some(std::time::Instant::now());
                    // Continue loop to receive final transcription
                }
            }
        }

        // Try to read WebSocket messages (non-blocking)
        match ws.read() {
            Ok(Message::Text(text)) => {
                eprintln!("Soniox response: {}", text);
                if let Ok(response) = serde_json::from_str::<SonioxResponse>(&text) {
                    if let Some(error) = response.error {
                        return Err(error);
                    }

                    if let Some(tokens) = response.tokens {
                        // Build transcript from tokens
                        interim_transcript.clear();
                        for token in &tokens {
                            if token.is_final {
                                final_transcript.push_str(&token.text);
                            } else {
                                interim_transcript.push_str(&token.text);
                            }
                        }

                        // Update live transcript (final + interim)
                        let display = format!("{}{}", final_transcript, interim_transcript);
                        if let Ok(mut t) = live_transcript.lock() {
                            *t = display.clone();
                        }

                        // Send update event
                        let _ = result_tx.send(VoiceCommandEvent::TranscriptUpdate {
                            text: display,
                            is_final: false,
                        });
                    }

                    if response.finished == Some(true) {
                        // Transcription complete
                        let final_text = format!("{}{}", final_transcript, interim_transcript);
                        let _ = result_tx.send(VoiceCommandEvent::TranscriptionComplete {
                            transcript: final_text,
                        });
                        return Ok(());
                    }
                }
            }
            Ok(Message::Close(_)) => {
                // Server closed connection
                let final_text = format!("{}{}", final_transcript, interim_transcript);
                let _ = result_tx.send(VoiceCommandEvent::TranscriptionComplete {
                    transcript: final_text,
                });
                return Ok(());
            }
            Err(tungstenite::Error::Io(ref e)) if e.kind() == std::io::ErrorKind::WouldBlock => {
                // No data available, sleep briefly and continue
                thread::sleep(std::time::Duration::from_millis(10));
            }
            Err(e) => {
                // Check if it's a close frame or real error
                if matches!(e, tungstenite::Error::ConnectionClosed | tungstenite::Error::AlreadyClosed) {
                    let final_text = format!("{}{}", final_transcript, interim_transcript);
                    let _ = result_tx.send(VoiceCommandEvent::TranscriptionComplete {
                        transcript: final_text,
                    });
                    return Ok(());
                }
                return Err(format!("WebSocket read error: {}", e));
            }
            _ => {}
        }
    }

    // Send final transcript
    let final_text = format!("{}{}", final_transcript, interim_transcript);
    if !final_text.is_empty() {
        let _ = result_tx.send(VoiceCommandEvent::TranscriptionComplete {
            transcript: final_text,
        });
    }

    Ok(())
}

/// Transcribe audio (legacy batch mode - now unused, kept for compatibility)
pub fn transcribe_audio(
    _samples: &[f32],
    _sample_rate: u32,
    _config: &VoiceCommandConfig,
    result_tx: Sender<VoiceCommandEvent>,
) {
    // Real-time transcription is now used instead
    // This function is called when recording ends, but with real-time mode
    // the transcript should already be available
    thread::spawn(move || {
        let _ = result_tx.send(VoiceCommandEvent::TranscriptionFailed {
            error: "Use real-time transcription instead".to_string(),
        });
    });
}

// ============================================================================
// LLM Integration
// ============================================================================

/// LLM API request/response structures
#[derive(Debug, Serialize)]
struct LlmRequest {
    model: String,
    messages: Vec<LlmMessage>,
    tools: Vec<LlmTool>,
    max_tokens: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct LlmMessage {
    role: String,
    #[serde(default)]
    content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<LlmToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_call_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct LlmToolCall {
    id: String,
    #[serde(rename = "type")]
    call_type: String,
    function: LlmToolFunction,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct LlmToolFunction {
    name: String,
    arguments: String,
}

#[derive(Debug, Clone, Serialize)]
struct LlmTool {
    #[serde(rename = "type")]
    tool_type: String,
    function: LlmToolDefinition,
}

#[derive(Debug, Clone, Serialize)]
struct LlmToolDefinition {
    name: String,
    description: String,
    parameters: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct LlmResponse {
    choices: Vec<LlmChoice>,
}

#[derive(Debug, Deserialize)]
struct LlmChoice {
    message: LlmMessage,
    finish_reason: Option<String>,
}

/// Build the system prompt for the LLM
fn build_system_prompt(context: &str, mime_type: &str, direction: &str) -> String {
    format!(r#"Voice command interpreter for Gradesta Browser, a spatial graph editor.

## Gradesta Concepts
- **Cells/Vertices**: Nodes in a graph containing text, audio, images, or other content
- **Navigation**: Move between cells using directions (north/south/east/west/up/down)
- **Bag**: A clipboard stack. "Yank" copies current cell to bag, "paste" connects bag top to current cell
- **Elves**: External AI agents that can generate content (images, audio, etc.)
- **Connecting**: Enter a server URL and refresh to connect to a Gradesta server

## Tools
- get_commands(category): Discover available commands by category
- get_services(): Get local servers and elves the user has configured
- request_view(targets, reason): Ask permission to view cell content

## Response Format
Return ONLY a raw JSON array (no markdown, no code blocks). Sorted by confidence:
[{{"script": "command.slug\nanother.command", "confidence": 0.9, "explanation": "Brief description"}}]

## Script Syntax
- Newline-separated command slugs (use \n in JSON)
- Use EXACT slugs from get_commands() output
- Special: `insert_text "content"` sets text AND auto-submits (no separate submit needed)

## IMPORTANT: Command Order
Commands execute in order. Some commands clear buffers, so order matters!
- graph.new_text_vertex CLEARS the text buffer, then enters text input mode
- insert_text "content" sets the text buffer AND automatically submits it

Example: "create a note saying hello to the east"
CORRECT (2 commands only):
graph.set_direction_east
graph.new_text_vertex
insert_text "hello"

WRONG order (buffer gets cleared):
insert_text "hello"
graph.new_text_vertex  <-- this clears the buffer!

If no match: {{"script": "", "confidence": 1.0, "explanation": "Could not understand"}}

Context: {context} | MIME: {mime_type} | Direction: {direction}
"#, context = context, mime_type = mime_type, direction = direction)
}

/// Build the tools array for the LLM
fn build_tools() -> Vec<LlmTool> {
    vec![
        LlmTool {
            tool_type: "function".to_string(),
            function: LlmToolDefinition {
                name: "get_commands".to_string(),
                description: "Get available commands for a category".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "category": {
                            "type": "string",
                            "enum": ["navigation", "editing", "clipboard", "ui", "text_input", "recording", "export", "elf"],
                            "description": "Command category to retrieve"
                        }
                    },
                    "required": ["category"]
                }),
            },
        },
        LlmTool {
            tool_type: "function".to_string(),
            function: LlmToolDefinition {
                name: "get_services".to_string(),
                description: "Get available local services (servers and elves) that can be connected to".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {},
                    "required": []
                }),
            },
        },
        LlmTool {
            tool_type: "function".to_string(),
            function: LlmToolDefinition {
                name: "request_view".to_string(),
                description: "Request permission to view cell content".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "targets": {
                            "type": "array",
                            "items": {
                                "type": "string",
                                "enum": ["current", "north", "south", "east", "west", "up", "down"]
                            },
                            "description": "Which cells to view"
                        },
                        "reason": {
                            "type": "string",
                            "description": "Why you need to see the content"
                        }
                    },
                    "required": ["targets", "reason"]
                }),
            },
        },
    ]
}

/// Get commands for a category by querying the Command enum
fn get_commands_for_category(category: &str) -> serde_json::Value {
    use crate::commands::Context;

    let target_context = match category {
        "navigation" | "editing" | "clipboard" => Some(Context::Graph),
        "ui" => Some(Context::Global),
        "text_input" => Some(Context::TextInput),
        "recording" => Some(Context::Recording),
        "export" => Some(Context::Export),
        "elf" => Some(Context::Elf),
        "bag" => Some(Context::Bag),
        _ => None,
    };

    let commands: Vec<serde_json::Value> = Command::all()
        .into_iter()
        .filter(|cmd| {
            if let Some(ctx) = target_context {
                cmd.context() == ctx
            } else {
                false
            }
        })
        .map(|cmd| {
            serde_json::json!({
                "slug": cmd.slug(),
                "description": cmd.description(),
                "phrases": cmd.voice_phrases()
            })
        })
        .collect();

    serde_json::json!({ "commands": commands })
}

/// Get available local services (servers and elves)
fn get_available_services() -> serde_json::Value {
    use crate::local_services::LocalServices;

    let services = LocalServices::load();

    match services {
        Some(ls) => {
            serde_json::json!({
                "servers": ls.servers.iter().map(|s| {
                    serde_json::json!({
                        "name": s.name,
                        "url": s.url
                    })
                }).collect::<Vec<_>>(),
                "elves": ls.elves.iter().map(|e| {
                    serde_json::json!({
                        "name": e.name,
                        "url": e.url
                    })
                }).collect::<Vec<_>>(),
                "instructions": "To connect to a server, use insert_text with target='url_bar' and content=<server url>"
            })
        }
        None => {
            serde_json::json!({
                "servers": [],
                "elves": [],
                "error": "No local services configuration found"
            })
        }
    }
}

/// Query the LLM with the voice command transcript
pub fn query_llm(
    transcript: &str,
    context: &str,
    mime_type: &str,
    direction: &str,
    cell_context: Option<&CellContext>,
    api_key: &str,
    config: &VoiceCommandConfig,
    result_tx: Sender<VoiceCommandEvent>,
) {
    let transcript = transcript.to_string();
    let context = context.to_string();
    let mime_type = mime_type.to_string();
    let direction = direction.to_string();
    let cell_context = cell_context.cloned();
    let api_key = api_key.to_string();
    let model = config.model.clone();

    thread::spawn(move || {
        match query_llm_sync(&transcript, &context, &mime_type, &direction, cell_context.as_ref(), &api_key, &model) {
            Ok(LlmResult::Interpretations(interpretations)) => {
                let _ = result_tx.send(VoiceCommandEvent::LlmResponse { interpretations });
            }
            Ok(LlmResult::RequestView { targets, reason }) => {
                let _ = result_tx.send(VoiceCommandEvent::LlmRequestsView { targets, reason });
            }
            Err(e) => {
                let _ = result_tx.send(VoiceCommandEvent::LlmFailed { error: e });
            }
        }
    });
}

enum LlmResult {
    Interpretations(Vec<AgentInterpretation>),
    RequestView { targets: Vec<String>, reason: String },
}

fn query_llm_sync(
    transcript: &str,
    context: &str,
    mime_type: &str,
    direction: &str,
    cell_context: Option<&CellContext>,
    api_key: &str,
    model: &str,
) -> Result<LlmResult, String> {
    let system_prompt = build_system_prompt(context, mime_type, direction);

    // Build user message
    let mut user_message = format!("User said: \"{}\"", transcript);

    // Add cell context if provided
    if let Some(ctx) = cell_context {
        user_message.push_str("\n\n## Cell Content (user granted access)");
        if let Some(ref current) = ctx.current {
            user_message.push_str(&format!("\nCurrent: \"{}\"", current));
        }
        for (dir, content) in &ctx.directions {
            if content.is_empty() {
                user_message.push_str(&format!("\n{}: (empty)", dir));
            } else {
                user_message.push_str(&format!("\n{}: \"{}\"", dir, content));
            }
        }
        user_message.push_str("\n\nNow build the final menu of actions.");
    }

    let mut messages = vec![
        LlmMessage {
            role: "system".to_string(),
            content: system_prompt,
            tool_calls: None,
            tool_call_id: None,
        },
        LlmMessage {
            role: "user".to_string(),
            content: user_message,
            tool_calls: None,
            tool_call_id: None,
        },
    ];

    let tools = build_tools();
    let client = reqwest::blocking::Client::new();

    // Loop to handle tool calls
    for _iteration in 0..5 {
        let request = LlmRequest {
            model: model.to_string(),
            messages: messages.clone(),
            tools: tools.clone(),
            max_tokens: 1024,
        };

        let response = client
            .post("https://router.requesty.ai/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .map_err(|e| format!("LLM request failed: {}", e))?;

        if !response.status().is_success() {
            let error_text = response.text().unwrap_or_default();
            return Err(format!("LLM API error: {}", error_text));
        }

        let response_text = response.text().map_err(|e| format!("Failed to read LLM response: {}", e))?;
        eprintln!("LLM raw response: {}", &response_text[..response_text.len().min(500)]);

        let llm_response: LlmResponse = serde_json::from_str(&response_text)
            .map_err(|e| format!("Failed to parse LLM response: {}", e))?;

        let choice = llm_response.choices.into_iter().next()
            .ok_or("No response from LLM")?;

        eprintln!("LLM content: {:?}", &choice.message.content);
        eprintln!("LLM tool_calls: {:?}", &choice.message.tool_calls);

        // Check for tool calls
        if let Some(tool_calls) = choice.message.tool_calls {
            // Add assistant message with tool calls
            messages.push(LlmMessage {
                role: "assistant".to_string(),
                content: choice.message.content.clone(),
                tool_calls: Some(tool_calls.clone()),
                tool_call_id: None,
            });

            // Process each tool call
            for tool_call in tool_calls {
                let args: serde_json::Value = serde_json::from_str(&tool_call.function.arguments)
                    .unwrap_or(serde_json::Value::Null);

                match tool_call.function.name.as_str() {
                    "get_commands" => {
                        let category = args["category"].as_str().unwrap_or("");
                        let result = get_commands_for_category(category);
                        messages.push(LlmMessage {
                            role: "tool".to_string(),
                            content: result.to_string(),
                            tool_calls: None,
                            tool_call_id: Some(tool_call.id),
                        });
                    }
                    "get_services" => {
                        let result = get_available_services();
                        messages.push(LlmMessage {
                            role: "tool".to_string(),
                            content: result.to_string(),
                            tool_calls: None,
                            tool_call_id: Some(tool_call.id),
                        });
                    }
                    "request_view" => {
                        // LLM wants to see cell content - return to caller for permission
                        let targets: Vec<String> = args["targets"]
                            .as_array()
                            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                            .unwrap_or_default();
                        let reason = args["reason"].as_str().unwrap_or("").to_string();
                        return Ok(LlmResult::RequestView { targets, reason });
                    }
                    _ => {
                        messages.push(LlmMessage {
                            role: "tool".to_string(),
                            content: "Unknown tool".to_string(),
                            tool_calls: None,
                            tool_call_id: Some(tool_call.id),
                        });
                    }
                }
            }
            // Continue loop to get final response
            continue;
        }

        // No tool calls - parse final response
        let content = choice.message.content;
        return parse_llm_response(&content);
    }

    Err("LLM exceeded maximum iterations".to_string())
}

fn parse_llm_response(content: &str) -> Result<LlmResult, String> {
    // Handle empty content
    if content.trim().is_empty() {
        eprintln!("LLM returned empty response, returning 'no match' interpretation");
        return Ok(LlmResult::Interpretations(vec![AgentInterpretation {
            action: AgentAction::Cancel,
            confidence: 1.0,
            explanation: "LLM returned empty response - try rephrasing your command".to_string(),
        }]));
    }

    // Try to find JSON array in the response
    let json_start = content.find('[');
    let json_end = content.rfind(']');

    if let (Some(start), Some(end)) = (json_start, json_end) {
        if end > start {
            let json_str = &content[start..=end];
            let options: Vec<serde_json::Value> = serde_json::from_str(json_str)
                .map_err(|e| format!("Failed to parse LLM JSON: {}", e))?;

            let interpretations: Vec<AgentInterpretation> = options
                .into_iter()
                .filter_map(|v| {
                    let confidence = v["confidence"].as_f64().unwrap_or(0.5) as f32;
                    let explanation = v["explanation"].as_str().unwrap_or("").to_string();
                    let script = v["script"].as_str().unwrap_or("").to_string();

                    // Empty script means cancel/no match
                    let action = if script.is_empty() {
                        AgentAction::Cancel
                    } else {
                        AgentAction::Script(script)
                    };

                    Some(AgentInterpretation {
                        action,
                        confidence,
                        explanation,
                    })
                })
                .collect();

            if interpretations.is_empty() {
                return Ok(LlmResult::Interpretations(vec![AgentInterpretation {
                    action: AgentAction::Cancel,
                    confidence: 1.0,
                    explanation: "Could not parse any actions from LLM response".to_string(),
                }]));
            }

            return Ok(LlmResult::Interpretations(interpretations));
        }
    }

    // Return a "no match" interpretation instead of error for missing JSON
    eprintln!("Could not find JSON array in LLM response: {}", content);
    Ok(LlmResult::Interpretations(vec![AgentInterpretation {
        action: AgentAction::Cancel,
        confidence: 1.0,
        explanation: "Could not understand response - try rephrasing your command".to_string(),
    }]))
}

// ============================================================================
// Script Parsing
// ============================================================================

/// A single parsed instruction from a script
#[derive(Debug, Clone)]
pub enum ScriptInstruction {
    /// Execute a command by slug (e.g., "graph.navigate_north")
    Command(Command),
    /// Insert text into the current text buffer (only special case needed)
    InsertText(String),
}

/// Parse a script string into individual instructions
/// Script format: newline-separated command slugs
/// Special: `insert_text "content"` sets text buffer before text_input commands
pub fn parse_script(script: &str) -> Vec<ScriptInstruction> {
    let mut instructions = Vec::new();

    for line in script.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // Special case: insert_text "content" - sets text buffer
        if line.starts_with("insert_text ") {
            let content = line[12..].trim().trim_matches('"');
            instructions.push(ScriptInstruction::InsertText(content.to_string()));
        } else if let Some(cmd) = Command::from_slug(line) {
            instructions.push(ScriptInstruction::Command(cmd));
        } else {
            eprintln!("Unknown command in script: {}", line);
        }
    }

    instructions
}
