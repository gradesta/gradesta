//! Claude Code Elf implementation

use async_trait::async_trait;
use gradesta_elf::{Elf, ElfCommand, ElfContext, ElfManifest};
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

        // TODO: Actually read the vertex content from the graph
        // For now, use the prompt parameter as config
        let config_text = ctx.param("config").cloned()
            .unwrap_or_else(|| r#"
instruction = "Default Claude Code configuration"
workdir = "/home/user"
"#.to_string());

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

        // Build prompt from region content or instruction
        let prompt = ctx.param("prompt").cloned()
            .unwrap_or_else(|| {
                if !config.instruction.is_empty() {
                    config.instruction.clone()
                } else {
                    "Help me with this code".to_string()
                }
            });

        ctx.output_line(&format!("Prompt: {}", prompt)).await?;

        let workdir = config.effective_workdir();
        ctx.output_line(&format!("Working directory: {}", workdir.display())).await?;

        // Check if claude is available
        if which::which("claude").is_err() {
            ctx.output_line("Error: 'claude' command not found in PATH").await?;
            ctx.output_line("Please install Claude Code CLI first.").await?;
            ctx.complete(1, "claude not found").await?;
            return Ok(());
        }

        // Run Claude Code
        ctx.output_line("Running Claude Code...").await?;

        use std::process::Stdio;
        use tokio::io::{AsyncBufReadExt, BufReader};
        use tokio::process::Command;

        let mut cmd = Command::new("claude");
        cmd.arg(&prompt);
        cmd.current_dir(&workdir);
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let mut child = cmd.spawn()?;

        // Stream stdout
        if let Some(stdout) = child.stdout.take() {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();
            while let Some(line) = lines.next_line().await? {
                ctx.output_line(&line).await?;
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
