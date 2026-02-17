//! Docker container management for Claude Code execution
//!
//! Handles building and running Docker containers for Claude Code.

use anyhow::{anyhow, Result};
use std::path::Path;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

use crate::config::ClaudeCodeConfig;

/// Build a Docker image from a nix-shell
pub async fn build_nix_docker_image(
    nix_shell_path: &Path,
    image_name: &str,
    output_fn: impl Fn(&str) + Send,
) -> Result<()> {
    output_fn(&format!("Building Docker image from {}\n", nix_shell_path.display()));

    // Create a Dockerfile that uses nix-shell and installs Claude Code
    let dockerfile_content = format!(
        r#"FROM nixos/nix:latest

# Copy shell.nix
COPY {} /shell.nix

# Update channels and build nix environment
RUN nix-channel --update

# Install dependencies from shell.nix
RUN nix-shell /shell.nix --run "echo Dependencies installed"

# Set up npm global directory and install Claude Code
ENV NPM_CONFIG_PREFIX=/root/.npm-global
ENV PATH="/root/.npm-global/bin:$PATH"
RUN mkdir -p /root/.npm-global
RUN nix-shell /shell.nix --run "npm install -g @anthropic-ai/claude-code"

# Verify claude is installed
RUN nix-shell /shell.nix --run "claude --version"

WORKDIR /workspace
ENTRYPOINT ["nix-shell", "/shell.nix", "--run"]
"#,
        nix_shell_path.file_name().unwrap().to_string_lossy()
    );

    // Get the directory containing the nix-shell
    let context_dir = nix_shell_path.parent()
        .ok_or_else(|| anyhow!("Invalid nix-shell path"))?;

    // Write temporary Dockerfile
    let dockerfile_path = context_dir.join("Dockerfile.claude-code-elf");
    tokio::fs::write(&dockerfile_path, &dockerfile_content).await?;

    // Build the image
    let mut cmd = Command::new("docker");
    cmd.args(["build", "-t", image_name, "-f"])
        .arg(&dockerfile_path)
        .arg(context_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = cmd.spawn()?;

    // Stream stdout
    if let Some(stdout) = child.stdout.take() {
        let reader = BufReader::new(stdout);
        let mut lines = reader.lines();
        while let Some(line) = lines.next_line().await? {
            output_fn(&format!("{}\n", line));
        }
    }

    let status = child.wait().await?;

    // Clean up Dockerfile
    let _ = tokio::fs::remove_file(&dockerfile_path).await;

    if !status.success() {
        return Err(anyhow!("Docker build failed with status: {}", status));
    }

    output_fn("Docker image built successfully\n");
    Ok(())
}

/// Run Claude Code in a Docker container
pub async fn run_claude_code_docker(
    config: &ClaudeCodeConfig,
    image_name: &str,
    prompt: &str,
    output_fn: impl Fn(&str) + Send,
) -> Result<()> {
    let workdir = config.effective_workdir();
    let claude_home = dirs::home_dir()
        .map(|h| h.join(".claude"))
        .ok_or_else(|| anyhow!("Could not determine home directory"))?;

    output_fn(&format!("Running Claude Code in Docker container: {}\n", image_name));
    output_fn(&format!("Working directory: {}\n", workdir.display()));

    let mut cmd = Command::new("docker");
    cmd.args(["run", "--rm", "-i"]);

    // Mount working directory
    cmd.arg("-v")
        .arg(format!("{}:/workspace", workdir.display()));

    // Mount Claude credentials (read-only)
    if claude_home.exists() {
        cmd.arg("-v")
            .arg(format!("{}:/root/.claude:ro", claude_home.display()));
    }

    // Set working directory
    cmd.args(["-w", "/workspace"]);

    // Add environment variables
    for (key, value) in &config.env {
        cmd.arg("-e").arg(format!("{}={}", key, value));
    }

    // Image and command
    cmd.arg(image_name);

    // The entrypoint is nix-shell --run, so we just pass the command
    let claude_cmd = format!("claude-code {}", shell_escape::escape(prompt.into()));
    cmd.arg(&claude_cmd);

    cmd.stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = cmd.spawn()?;

    // Stream output
    if let Some(stdout) = child.stdout.take() {
        let reader = BufReader::new(stdout);
        let mut lines = reader.lines();
        while let Some(line) = lines.next_line().await? {
            output_fn(&format!("{}\n", line));
        }
    }

    let status = child.wait().await?;

    if !status.success() {
        output_fn(&format!("Claude Code exited with status: {}\n", status));
    } else {
        output_fn("Claude Code completed successfully\n");
    }

    Ok(())
}

/// Run Claude Code directly (without Docker)
pub async fn run_claude_code_direct(
    config: &ClaudeCodeConfig,
    prompt: &str,
    output_fn: impl Fn(&str) + Send,
) -> Result<()> {
    let workdir = config.effective_workdir();

    output_fn(&format!("Running Claude Code in {}\n", workdir.display()));

    // Check if claude-code is available
    if which::which("claude-code").is_err() {
        return Err(anyhow!("claude-code not found in PATH"));
    }

    let mut cmd = Command::new("claude-code");
    cmd.arg(prompt);
    cmd.current_dir(&workdir);

    // Add environment variables
    for (key, value) in &config.env {
        cmd.env(key, value);
    }

    cmd.stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = cmd.spawn()?;

    // Stream output
    if let Some(stdout) = child.stdout.take() {
        let reader = BufReader::new(stdout);
        let mut lines = reader.lines();
        while let Some(line) = lines.next_line().await? {
            output_fn(&format!("{}\n", line));
        }
    }

    let status = child.wait().await?;

    if !status.success() {
        output_fn(&format!("Claude Code exited with status: {}\n", status));
    } else {
        output_fn("Claude Code completed successfully\n");
    }

    Ok(())
}
