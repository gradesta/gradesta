//! End-to-end integration test for the Elves system
//!
//! Tests the full flow:
//! 1. Start test-server
//! 2. Start pig-latin-elf
//! 3. Connect headless browser
//! 4. Create a vertex with text content
//! 5. Introduce the elf
//! 6. Summon the elf to transform the content
//! 7. Verify the content was transformed to pig latin

use anyhow::{anyhow, Result};
use futures_util::{SinkExt, StreamExt};
use std::collections::HashMap;
use std::process::Stdio;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::time::timeout;
use tokio_tungstenite::{connect_async, tungstenite::Message};

// Protocol constants
const MSG_CLIENT_WATCH_LANDMARK: u8 = 0x81;
const MSG_CLIENT_CREATE_VERTEX: u8 = 0x86;
const MSG_CLIENT_INTRODUCE_ELF: u8 = 0xA0;
const MSG_SERVER_SET_CONTEXT: u8 = 0x01;
const MSG_SERVER_SET_EDGES: u8 = 0x03;
const MSG_SERVER_SET_VERTEX_LABEL: u8 = 0x05;
const MSG_SERVER_LOG_MESSAGE: u8 = 0x0F;
const MSG_SERVER_INTRODUCTION_TOKEN: u8 = 0x20;

const SERVER_PORT: u16 = 8098;
const ELF_PORT: u16 = 9098;

struct TestHarness {
    server: Child,
    elf: Child,
    storage_dir: tempfile::TempDir,
}

impl Drop for TestHarness {
    fn drop(&mut self) {
        // Kill processes on drop
        let _ = self.server.start_kill();
        let _ = self.elf.start_kill();
    }
}

async fn start_server(storage_dir: &std::path::Path) -> Result<Child> {
    let child = Command::new("cargo")
        .args([
            "run",
            "--",
            "--port",
            &SERVER_PORT.to_string(),
            "--storage",
            storage_dir.to_str().unwrap(),
        ])
        .current_dir("../test-server")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    // Wait for server to be ready (retry connecting)
    for i in 0..60 {
        tokio::time::sleep(Duration::from_millis(500)).await;
        if tokio_tungstenite::connect_async(format!("ws://localhost:{}/ws", SERVER_PORT)).await.is_ok() {
            println!("  Server ready after {}ms", (i + 1) * 500);
            return Ok(child);
        }
    }
    Err(anyhow!("Server failed to start within 30 seconds"))
}

async fn start_elf() -> Result<Child> {
    let child = Command::new("cargo")
        .args([
            "run",
            "--",
            "--port",
            &ELF_PORT.to_string(),
        ])
        .current_dir("../pig-latin-elf")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    // Wait for elf to be ready (retry checking manifest)
    let client = reqwest::Client::new();
    for i in 0..60 {
        tokio::time::sleep(Duration::from_millis(500)).await;
        if client.get(format!("http://localhost:{}/manifest.json", ELF_PORT))
            .send()
            .await
            .is_ok() {
            println!("  Elf ready after {}ms", (i + 1) * 500);
            return Ok(child);
        }
    }
    Err(anyhow!("Elf failed to start within 30 seconds"))
}

// Protocol encoding functions
fn encode_watch_landmark(action_id: u64, landmark: &str) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.push(MSG_CLIENT_WATCH_LANDMARK);
    buf.extend_from_slice(&action_id.to_be_bytes());
    buf.extend_from_slice(landmark.as_bytes());
    buf
}

fn encode_create_vertex(
    action_id: u64,
    from_vertex: u64,
    direction: u8,
    layer: u32,
    mime: &str,
    content: &[u8],
) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.push(MSG_CLIENT_CREATE_VERTEX);
    buf.extend_from_slice(&action_id.to_be_bytes());
    buf.extend_from_slice(&from_vertex.to_be_bytes());
    buf.push(direction);
    buf.extend_from_slice(&layer.to_be_bytes());
    buf.extend_from_slice(mime.as_bytes());
    buf.push(0);
    buf.extend_from_slice(content);
    buf
}

fn encode_introduce_elf(
    action_id: u64,
    elf_url: &str,
    command: &str,
    cursor_landmark: &str,
    cursor_vertex: u64,
    permissions: u8,
) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.push(MSG_CLIENT_INTRODUCE_ELF);
    buf.extend_from_slice(&action_id.to_be_bytes());
    buf.extend_from_slice(elf_url.as_bytes());
    buf.push(0);
    buf.extend_from_slice(command.as_bytes());
    buf.push(0);
    buf.extend_from_slice(cursor_landmark.as_bytes());
    buf.push(0);
    buf.extend_from_slice(&cursor_vertex.to_be_bytes());
    // region spec (use cursor as origin)
    buf.extend_from_slice(cursor_landmark.as_bytes());
    buf.push(0);
    buf.extend_from_slice(&cursor_vertex.to_be_bytes());
    buf.push(0xFF); // all directions
    buf.extend_from_slice(&(-1i32).to_be_bytes()); // unlimited depth
    buf.push(permissions);
    buf.extend_from_slice(&0u16.to_be_bytes()); // no params
    buf
}

#[derive(Debug)]
enum ServerMessage {
    Context { action_id: u64, landmark: String },
    Vertex { action_id: u64, vertex_id: u64, mime: String, content: Vec<u8> },
    Edges { action_id: u64, vertex_id: u64, edges: [u64; 6] },
    Log { action_id: u64, status: u32, vertex_id: u64, message: String },
    IntroductionToken { action_id: u64, token: String, server_ws_url: String },
    Unknown(u8),
}

fn parse_message(data: &[u8]) -> Result<ServerMessage> {
    if data.is_empty() {
        return Err(anyhow!("Empty message"));
    }

    match data[0] {
        MSG_SERVER_SET_CONTEXT => {
            let action_id = u64::from_be_bytes(data[1..9].try_into()?);
            let landmark = String::from_utf8(data[9..].to_vec())?;
            Ok(ServerMessage::Context { action_id, landmark })
        }
        MSG_SERVER_SET_VERTEX_LABEL => {
            let action_id = u64::from_be_bytes(data[1..9].try_into()?);
            let vertex_id = u64::from_be_bytes(data[9..17].try_into()?);
            let _layer = u32::from_be_bytes(data[17..21].try_into()?);
            let rest = &data[21..];
            let null_pos = rest.iter().position(|&b| b == 0).unwrap_or(rest.len());
            let mime = String::from_utf8(rest[..null_pos].to_vec())?;
            let content = rest[null_pos + 1..].to_vec();
            Ok(ServerMessage::Vertex { action_id, vertex_id, mime, content })
        }
        MSG_SERVER_SET_EDGES => {
            let action_id = u64::from_be_bytes(data[1..9].try_into()?);
            let vertex_id = u64::from_be_bytes(data[9..17].try_into()?);
            let mut edges = [0u64; 6];
            for i in 0..6 {
                let start = 17 + i * 8;
                edges[i] = u64::from_be_bytes(data[start..start + 8].try_into()?);
            }
            Ok(ServerMessage::Edges { action_id, vertex_id, edges })
        }
        MSG_SERVER_LOG_MESSAGE => {
            let action_id = u64::from_be_bytes(data[1..9].try_into()?);
            let status = u32::from_be_bytes(data[9..13].try_into()?);
            let vertex_id = u64::from_be_bytes(data[13..21].try_into()?);
            let message = String::from_utf8(data[21..].to_vec())?;
            Ok(ServerMessage::Log { action_id, status, vertex_id, message })
        }
        MSG_SERVER_INTRODUCTION_TOKEN => {
            let action_id = u64::from_be_bytes(data[1..9].try_into()?);
            let rest = &data[9..];
            let null_pos = rest.iter().position(|&b| b == 0).ok_or(anyhow!("No null"))?;
            let token = String::from_utf8(rest[..null_pos].to_vec())?;
            let rest = &rest[null_pos + 1..];
            let null_pos = rest.iter().position(|&b| b == 0).unwrap_or(rest.len());
            let server_ws_url = String::from_utf8(rest[..null_pos].to_vec())?;
            Ok(ServerMessage::IntroductionToken { action_id, token, server_ws_url })
        }
        t => Ok(ServerMessage::Unknown(t)),
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("Starting Elves integration test...\n");

    // Create temp storage directory
    let storage_dir = tempfile::tempdir()?;
    println!("Storage directory: {}", storage_dir.path().display());

    // Start server
    println!("Starting test server on port {}...", SERVER_PORT);
    let server = start_server(storage_dir.path()).await?;

    // Start elf
    println!("Starting pig-latin-elf on port {}...", ELF_PORT);
    let elf = start_elf().await?;

    let _harness = TestHarness {
        server,
        elf,
        storage_dir,
    };

    // Verify elf manifest is accessible
    println!("Checking elf manifest...");
    let client = reqwest::Client::new();
    let manifest: serde_json::Value = client
        .get(format!("http://localhost:{}/manifest.json", ELF_PORT))
        .send()
        .await?
        .json()
        .await?;
    assert_eq!(manifest["elf_id"], "pig-latin");
    println!("  Elf manifest OK: {}", manifest["name"]);

    // Connect to server
    println!("Connecting to server...");
    let (ws_stream, _) = connect_async(format!("ws://localhost:{}/ws", SERVER_PORT)).await?;
    let (mut write, mut read) = ws_stream.split();
    println!("  Connected!");

    // Watch landmark
    println!("Watching landmark...");
    let msg = encode_watch_landmark(1, "local://test");
    write.send(Message::Binary(msg)).await?;

    // Collect initial messages (context + possibly empty placeholder vertex)
    let mut action_id = 2u64;
    let mut vertex_ids: Vec<u64> = Vec::new();

    // Read initial messages
    loop {
        let msg = timeout(Duration::from_secs(2), read.next()).await;
        match msg {
            Ok(Some(Ok(Message::Binary(data)))) => {
                let parsed = parse_message(&data)?;
                println!("  Received: {:?}", parsed);
                match parsed {
                    ServerMessage::Vertex { vertex_id, .. } => {
                        vertex_ids.push(vertex_id);
                    }
                    ServerMessage::Edges { .. } => {
                        // After edges, initial sync is done
                        break;
                    }
                    _ => {}
                }
            }
            _ => break,
        }
    }

    // Create a vertex with test content
    let test_content = "hello world";
    println!("\nCreating vertex with content: \"{}\"", test_content);
    action_id += 1;
    let create_msg = encode_create_vertex(
        action_id,
        0, // from vertex 0 (root)
        3, // direction: south
        0, // layer 0
        "text/plain",
        test_content.as_bytes(),
    );
    write.send(Message::Binary(create_msg)).await?;

    // Wait for vertex creation response
    let created_vertex_id;
    loop {
        let msg = timeout(Duration::from_secs(5), read.next()).await?
            .ok_or(anyhow!("Connection closed"))??;

        if let Message::Binary(data) = msg {
            let parsed = parse_message(&data)?;
            println!("  Received: {:?}", parsed);
            match parsed {
                ServerMessage::Log { status, vertex_id, .. } if status == 200 => {
                    created_vertex_id = vertex_id;
                    println!("  Vertex created with ID: {}", vertex_id);
                    break;
                }
                _ => {}
            }
        }
    }

    // Introduce the elf
    println!("\nIntroducing elf...");
    action_id += 1;
    let elf_url = format!("http://localhost:{}", ELF_PORT);
    let introduce_msg = encode_introduce_elf(
        action_id,
        &elf_url,
        "transform",
        "local://test",
        created_vertex_id,
        0x03, // read + write
    );
    write.send(Message::Binary(introduce_msg)).await?;

    // Wait for introduction token
    let (token, server_ws_url);
    loop {
        let msg = timeout(Duration::from_secs(5), read.next()).await?
            .ok_or(anyhow!("Connection closed"))??;

        if let Message::Binary(data) = msg {
            let parsed = parse_message(&data)?;
            println!("  Received: {:?}", parsed);
            if let ServerMessage::IntroductionToken { token: t, server_ws_url: url, .. } = parsed {
                token = t;
                server_ws_url = url;
                break;
            }
        }
    }
    println!("  Got introduction token");

    // Summon the elf by POSTing to /summon
    println!("\nSummoning elf...");
    let summon_body = serde_json::json!({
        "token": token,
        "server_ws_url": server_ws_url,
        "command": "transform",
        "params": {}
    });
    let summon_resp = client
        .post(format!("http://localhost:{}/summon", ELF_PORT))
        .json(&summon_body)
        .send()
        .await?;
    assert!(summon_resp.status().is_success());
    println!("  Elf summoned!");

    // Wait for the elf to complete
    // The elf will read the vertex, transform it, and write it back
    // We can verify by reading the file from disk
    println!("\nWaiting for elf to complete...");
    tokio::time::sleep(Duration::from_secs(3)).await;

    // Read the content file from storage to verify transformation
    // Files are stored as content/{uuid}.txt
    let storage_path = _harness.storage_dir.path();
    let content_dir = storage_path.join("content");

    // Find the text file
    let mut transformed_content = String::new();
    if content_dir.exists() {
        for entry in std::fs::read_dir(&content_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().map(|e| e == "txt").unwrap_or(false) {
                transformed_content = std::fs::read_to_string(&path)?;
                println!("  Read file: {}", path.display());
                println!("  Content: \"{}\"", transformed_content);
                break;
            }
        }
    }

    if transformed_content.is_empty() {
        // Try index.toml to find the file path
        let index_path = storage_path.join("index.toml");
        println!("  Checking index.toml: {}", index_path.display());
        if index_path.exists() {
            let index_content = std::fs::read_to_string(&index_path)?;
            println!("  Index:\n{}", index_content);
        }
        return Err(anyhow!("Could not find content file"));
    }

    // Verify the transformation
    let expected = "ellohay orldway";
    println!("\nVerifying transformation...");
    println!("  Original: \"{}\"", test_content);
    println!("  Expected: \"{}\"", expected);
    println!("  Got:      \"{}\"", transformed_content);

    if transformed_content == expected {
        println!("\n========================================");
        println!("       INTEGRATION TEST PASSED!");
        println!("========================================\n");
        Ok(())
    } else {
        Err(anyhow!("Transformation mismatch! Expected '{}', got '{}'", expected, transformed_content))
    }
}
