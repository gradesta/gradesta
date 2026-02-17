//! Claude Code Elf implementation

use async_trait::async_trait;
use gradesta_elf::{Elf, ElfCommand, ElfContext, ElfManifest};
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::config::ClaudeCodeConfig;

/// Claude Code Elf
pub struct ClaudeCodeElf {
    /// Cached configuration (from read-config command)
    config: Arc<Mutex<Option<ClaudeCodeConfig>>>,
}

impl ClaudeCodeElf {
    pub fn new() -> Self {
        Self {
            config: Arc::new(Mutex::new(None)),
        }
    }
}

/// Stream JSON message types from Claude Code
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
enum StreamMessage {
    /// System message (init, etc.)
    System { subtype: String },
    /// Assistant text output
    Assistant { message: AssistantMessage },
    /// Tool use
    #[serde(rename = "tool_use")]
    ToolUse { name: String },
    /// Tool result
    #[serde(rename = "tool_result")]
    ToolResult { name: String },
    /// Final result
    Result { subtype: String, result: Option<String> },
}

#[derive(Debug, Deserialize)]
struct AssistantMessage {
    content: Vec<ContentBlock>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
enum ContentBlock {
    Text { text: String },
    #[serde(other)]
    Other,
}

impl AssistantMessage {
    fn text(&self) -> String {
        self.content.iter()
            .filter_map(|block| match block {
                ContentBlock::Text { text } => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("")
    }
}

#[async_trait]
impl Elf for ClaudeCodeElf {
    fn manifest(&self) -> ElfManifest {
        ElfManifest {
            elf_id: "claude-code".to_string(),
            name: "Claude Code".to_string(),
            description: "AI-powered coding assistant using Claude".to_string(),
            commands: vec![
                ElfCommand {
                    name: "read-config".to_string(),
                    description: "Parse cursor cell as TOML configuration".to_string(),
                    inputs: vec!["cursor".to_string()],
                },
                ElfCommand {
                    name: "code".to_string(),
                    description: "Execute Claude Code with region content as prompt".to_string(),
                    inputs: vec!["cursor".to_string(), "region".to_string()],
                },
            ],
        }
    }

    async fn handle_summon(&self, ctx: ElfContext) -> anyhow::Result<()> {
        match ctx.command() {
            "read-config" => self.handle_read_config(&ctx).await,
            "code" => self.handle_code(&ctx).await,
            cmd => {
                ctx.output_line(&format!("Unknown command: {}", cmd)).await?;
                ctx.complete(1, "Unknown command").await?;
                Ok(())
            }
        }
    }
}

impl ClaudeCodeElf {
    async fn handle_read_config(&self, ctx: &ElfContext) -> anyhow::Result<()> {
        ctx.output_line("Reading configuration from cursor cell...").await?;

        // Read the actual vertex content
        let (mime, content) = ctx.read_cursor().await?;

        if !mime.starts_with("text/") {
            ctx.output_line(&format!("Cursor content is not text ({})", mime)).await?;
            ctx.complete(1, "Not text content").await?;
            return Ok(());
        }

        let config_text = String::from_utf8_lossy(&content);

        match ClaudeCodeConfig::parse(&config_text) {
            Ok(config) => {
                ctx.output_line("Configuration parsed successfully:").await?;
                ctx.output_line(&format!("  instruction: {}", config.instruction)).await?;
                if let Some(ref workdir) = config.workdir {
                    ctx.output_line(&format!("  workdir: {}", workdir.display())).await?;
                }
                if let Some(ref docker_image) = config.docker_image {
                    ctx.output_line(&format!("  docker-image: {}", docker_image)).await?;
                }
                if let Some(ref nix_shell) = config.nix_shell {
                    ctx.output_line(&format!("  nix-shell: {}", nix_shell.display())).await?;
                }

                // Store configuration
                {
                    let mut stored = self.config.lock().await;
                    *stored = Some(config);
                }

                ctx.complete(0, "Configuration loaded").await?;
            }
            Err(e) => {
                ctx.output_line(&format!("Failed to parse configuration: {}", e)).await?;
                ctx.complete(1, "Configuration error").await?;
            }
        }

        Ok(())
    }

    async fn handle_code(&self, ctx: &ElfContext) -> anyhow::Result<()> {
        ctx.output_line("Starting Claude Code...").await?;

        // Get stored configuration or use defaults
        let config = {
            let stored = self.config.lock().await;
            stored.clone().unwrap_or_default()
        };

        // Read the cursor cell content as the prompt
        let (mime, content) = ctx.read_cursor().await?;
        let prompt = if mime.starts_with("text/") {
            String::from_utf8_lossy(&content).to_string()
        } else {
            // Use instruction from config if cursor is not text
            if !config.instruction.is_empty() {
                config.instruction.clone()
            } else {
                ctx.output_line("Error: Cursor is not text and no instruction in config").await?;
                ctx.complete(1, "No prompt").await?;
                return Ok(());
            }
        };

        ctx.output_line(&format!("Prompt: {}", prompt.lines().next().unwrap_or("(empty)"))).await?;

        let workdir = config.effective_workdir();
        ctx.output_line(&format!("Working directory: {}", workdir.display())).await?;

        // Check if claude is available
        if which::which("claude").is_err() {
            ctx.output_line("Error: 'claude' command not found in PATH").await?;
            ctx.output_line("Please install Claude Code CLI first.").await?;
            ctx.complete(1, "claude not found").await?;
            return Ok(());
        }

        // Track the current southernmost cell (start at cursor)
        let mut current_south = ctx.cursor_vertex();

        // Run Claude Code with stream-json output
        ctx.output_line("Running Claude Code with streaming output...").await?;

        use std::process::Stdio;
        use tokio::io::{AsyncBufReadExt, BufReader};
        use tokio::process::Command;

        let mut cmd = Command::new("claude");
        cmd.arg("--print");
        cmd.arg("--verbose");
        cmd.arg("--output-format");
        cmd.arg("stream-json");
        cmd.arg(&prompt);
        cmd.current_dir(&workdir);
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let mut child = cmd.spawn()?;

        // Stream stdout and create cells
        if let Some(stdout) = child.stdout.take() {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();

            // Accumulate content for the current cell
            let mut current_content = String::new();
            let mut last_cell_id: Option<u64> = None;

            while let Some(line) = lines.next_line().await? {
                // Try to parse as stream-json
                if let Ok(msg) = serde_json::from_str::<StreamMessage>(&line) {
                    match msg {
                        StreamMessage::System { subtype } => {
                            // Log system messages but don't create cells for them
                            log::info!("System message: {}", subtype);
                        }
                        StreamMessage::Assistant { message } => {
                            // Accumulate assistant content
                            let text = message.text();
                            if !text.is_empty() {
                                current_content.push_str(&text);
                            }

                            // Create or update the assistant output cell
                            if let Some(cell_id) = last_cell_id {
                                // Update existing cell
                                if let Err(e) = ctx.set_vertex(cell_id, "text/markdown", current_content.as_bytes()).await {
                                    log::error!("Failed to update cell: {}", e);
                                }
                            } else {
                                // Create new cell
                                match ctx.create_south(current_south, "text/markdown", current_content.as_bytes()).await {
                                    Ok(new_id) => {
                                        current_south = new_id;
                                        last_cell_id = Some(new_id);
                                        log::info!("Created assistant cell: {}", new_id);
                                    }
                                    Err(e) => {
                                        log::error!("Failed to create cell: {}", e);
                                        ctx.output_line(&format!("Error creating cell: {}", e)).await?;
                                    }
                                }
                            }
                        }
                        StreamMessage::ToolUse { name } => {
                            // Create a tool use indicator cell
                            let cell_content = format!("Using tool: {}", name);
                            match ctx.create_south(current_south, "text/plain", cell_content.as_bytes()).await {
                                Ok(new_id) => {
                                    current_south = new_id;
                                    // Reset content accumulator for next assistant message
                                    current_content.clear();
                                    last_cell_id = None;
                                    log::info!("Created tool cell: {}", new_id);
                                }
                                Err(e) => {
                                    log::error!("Failed to create cell: {}", e);
                                }
                            }
                        }
                        StreamMessage::ToolResult { name } => {
                            // Tool result - could add details here
                            log::info!("Tool result: {}", name);
                        }
                        StreamMessage::Result { subtype, result } => {
                            // Final result
                            if let Some(result_text) = result {
                                let cell_content = format!("[{}]\n{}", subtype, result_text);
                                match ctx.create_south(current_south, "text/markdown", cell_content.as_bytes()).await {
                                    Ok(new_id) => {
                                        current_south = new_id;
                                        log::info!("Created result cell: {}", new_id);
                                    }
                                    Err(e) => {
                                        log::error!("Failed to create cell: {}", e);
                                    }
                                }
                            }
                        }
                    }
                } else {
                    // Not JSON, output as plain text
                    log::debug!("Non-JSON line: {}", line);
                }
            }
        }

        // Also capture stderr
        if let Some(stderr) = child.stderr.take() {
            let reader = BufReader::new(stderr);
            let mut lines = reader.lines();
            while let Some(line) = lines.next_line().await? {
                ctx.output_line(&format!("stderr: {}", line)).await?;
            }
        }

        let status = child.wait().await?;

        if status.success() {
            ctx.output_line("Claude Code completed successfully.").await?;
            ctx.complete(0, "Success").await?;
        } else {
            ctx.output_line(&format!("Claude Code exited with: {}", status)).await?;
            ctx.complete(1, "Claude Code failed").await?;
        }

        Ok(())
    }
}
