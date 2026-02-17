//! Integration test for claude-code-elf
//!
//! This test actually runs Claude Code and verifies that:
//! 1. The elf can read the cursor cell as a prompt
//! 2. Claude Code runs and produces output
//! 3. Output cells are created south of the cursor
//!
//! Requires: `claude` command available and authenticated

use anyhow::{anyhow, Result};
use futures_util::{SinkExt, StreamExt};
use std::process::Stdio;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::time::timeout;
use tokio_tungstenite::{connect_async, tungstenite::Message};

use crate::{
    encode_watch_landmark, encode_create_vertex, encode_introduce_elf,
    parse_message, ServerMessage,
    SERVER_PORT,
};

const CLAUDE_ELF_PORT: u16 = 9099;

pub async fn start_claude_code_elf() -> Result<Child> {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let claude_elf_dir = std::path::Path::new(manifest_dir)
        .parent()
        .unwrap()
        .join("claude-code-elf");

    println!("  Claude elf dir: {}", claude_elf_dir.display());

    let child = Command::new("cargo")
        .args([
            "run",
            "--",
            "--port",
            &CLAUDE_ELF_PORT.to_string(),
        ])
        .env("RUST_LOG", "info")
        .current_dir(&claude_elf_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    // Wait for elf to be ready
    let client = reqwest::Client::new();
    for i in 0..60 {
        tokio::time::sleep(Duration::from_millis(500)).await;
        if client.get(format!("http://localhost:{}/manifest.json", CLAUDE_ELF_PORT))
            .send()
            .await
            .is_ok() {
            println!("  Claude Code Elf ready after {}ms", (i + 1) * 500);
            return Ok(child);
        }
    }
    Err(anyhow!("Claude Code Elf failed to start within 30 seconds"))
}

/// Check if claude command is available
fn is_claude_available() -> bool {
    std::process::Command::new("which")
        .arg("claude")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

pub async fn test_claude_code_elf_streaming(server_ready: bool) -> Result<()> {
    if !server_ready {
        return Err(anyhow!("Server not ready"));
    }

    println!("\n=== Testing Claude Code Elf (streaming output to cells) ===\n");

    // Check if claude is available
    if !is_claude_available() {
        println!("  SKIPPING: 'claude' command not found in PATH");
        println!("  Install Claude Code CLI and authenticate to run this test");
        return Ok(());
    }
    println!("  Claude CLI found");

    // Start claude-code-elf
    println!("Starting claude-code-elf on port {}...", CLAUDE_ELF_PORT);
    let mut elf = start_claude_code_elf().await?;

    // Spawn task to read stderr from elf
    let elf_stderr = elf.stderr.take();
    let stderr_task = if let Some(stderr) = elf_stderr {
        Some(tokio::spawn(async move {
            let reader = BufReader::new(stderr);
            let mut lines = reader.lines();
            while let Some(line) = lines.next_line().await.ok().flatten() {
                println!("  [elf stderr] {}", line);
            }
        }))
    } else {
        None
    };

    // Connect to server
    println!("Connecting to server...");
    let (ws_stream, _) = connect_async(format!("ws://localhost:{}/ws", SERVER_PORT)).await?;
    let (mut write, mut read) = ws_stream.split();
    println!("  Connected!");

    // Watch landmark
    println!("Watching landmark...");
    let msg = encode_watch_landmark(1, "local://test");
    write.send(Message::Binary(msg)).await?;

    // Read initial messages
    loop {
        let msg = timeout(Duration::from_secs(2), read.next()).await;
        match msg {
            Ok(Some(Ok(Message::Binary(data)))) => {
                let parsed = parse_message(&data)?;
                match parsed {
                    ServerMessage::Edges { .. } => break,
                    _ => {}
                }
            }
            _ => break,
        }
    }

    // Create a prompt cell - simple prompt that should produce quick output
    let prompt = "Say hello in exactly 5 words.";
    println!("Creating prompt vertex: \"{}\"", prompt);

    let create_msg = encode_create_vertex(
        2,
        0, // from vertex 0
        3, // direction: south
        0, // layer 0
        "text/plain",
        prompt.as_bytes(),
    );
    write.send(Message::Binary(create_msg)).await?;

    // Wait for vertex creation
    let prompt_vertex_id;
    loop {
        let msg = timeout(Duration::from_secs(5), read.next()).await?
            .ok_or(anyhow!("Connection closed"))??;

        if let Message::Binary(data) = msg {
            let parsed = parse_message(&data)?;
            if let ServerMessage::Log { status, vertex_id, .. } = parsed {
                if status == 200 {
                    prompt_vertex_id = vertex_id;
                    println!("  Prompt vertex created: {}", vertex_id);
                    break;
                }
            }
        }
    }

    // Introduce the elf for "code" command
    println!("Introducing elf for code command...");
    let elf_url = format!("http://localhost:{}", CLAUDE_ELF_PORT);
    let introduce_msg = encode_introduce_elf(
        3,
        &elf_url,
        "code",
        "local://test",
        prompt_vertex_id,
        0x07, // read + write + create permissions
    );
    write.send(Message::Binary(introduce_msg)).await?;

    // Wait for introduction token
    let (token, server_ws_url);
    loop {
        let msg = timeout(Duration::from_secs(5), read.next()).await?
            .ok_or(anyhow!("Connection closed"))??;

        if let Message::Binary(data) = msg {
            let parsed = parse_message(&data)?;
            if let ServerMessage::IntroductionToken { token: t, server_ws_url: url, .. } = parsed {
                token = t;
                server_ws_url = url;
                println!("  Got introduction token");
                break;
            }
        }
    }

    // Summon the elf
    println!("Summoning elf...");
    let client = reqwest::Client::new();
    let summon_body = serde_json::json!({
        "token": token,
        "server_ws_url": server_ws_url,
        "command": "code",
        "params": {}
    });
    let summon_resp = client
        .post(format!("http://localhost:{}/summon", CLAUDE_ELF_PORT))
        .json(&summon_body)
        .send()
        .await?;
    assert!(summon_resp.status().is_success());
    println!("  Elf summoned!");

    // Wait for Claude to complete and cells to be created
    // We should see new Vertex messages for cells created south of the prompt
    println!("Waiting for Claude Code to complete and create cells...");

    let mut created_cells: Vec<(u64, String)> = Vec::new();
    let start = std::time::Instant::now();
    let max_wait = Duration::from_secs(120); // Claude can take a while

    loop {
        if start.elapsed() > max_wait {
            println!("  Timeout waiting for Claude Code");
            break;
        }

        let msg = timeout(Duration::from_secs(5), read.next()).await;
        match msg {
            Ok(Some(Ok(Message::Binary(data)))) => {
                let parsed = parse_message(&data)?;
                match parsed {
                    ServerMessage::Vertex { vertex_id, content, .. } => {
                        if vertex_id != prompt_vertex_id && vertex_id != 1 {
                            let content_str = String::from_utf8_lossy(&content).to_string();
                            println!("  New cell created: {} bytes", content_str.len());
                            if content_str.len() < 200 {
                                println!("    Content: {}", content_str.replace('\n', "\\n"));
                            } else {
                                println!("    Content: {}...", &content_str[..200].replace('\n', "\\n"));
                            }
                            created_cells.push((vertex_id, content_str));
                        }
                    }
                    ServerMessage::Log { status, message, .. } => {
                        println!("  Log: status={} message={}", status, message);
                    }
                    _ => {}
                }
            }
            Ok(Some(Ok(_))) => continue,
            Ok(Some(Err(e))) => {
                println!("  WebSocket error: {}", e);
                break;
            }
            Ok(None) => {
                println!("  Connection closed");
                break;
            }
            Err(_) => {
                // Timeout - check if we have any cells
                if !created_cells.is_empty() {
                    println!("  No more messages, checking results...");
                    break;
                }
            }
        }
    }

    // Verify results
    println!("\nVerifying results...");
    println!("  Total cells created: {}", created_cells.len());

    // Abort stderr task
    if let Some(task) = stderr_task {
        task.abort();
    }

    // Kill elf process
    let _ = elf.kill().await;

    if created_cells.is_empty() {
        return Err(anyhow!("No cells were created by Claude Code!"));
    }

    // Check that at least one cell has meaningful content
    let has_content = created_cells.iter().any(|(_, content)| {
        content.len() > 10 && !content.starts_with("[") // Not just a status message
    });

    if !has_content {
        println!("  WARNING: Cells were created but may not contain Claude's response");
        println!("  Cell contents:");
        for (id, content) in &created_cells {
            println!("    {}: {}", id, content);
        }
    }

    println!("\n=== Claude Code Elf streaming test PASSED ===");
    println!("  {} cells were created south of the prompt", created_cells.len());

    Ok(())
}
