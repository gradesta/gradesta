//! Configuration parsing for Claude Code
//!
//! Parses TOML configuration from graph cells.

use anyhow::{anyhow, Result};
use serde::Deserialize;
use std::path::PathBuf;

/// Claude Code configuration
#[derive(Clone, Debug, Deserialize)]
pub struct ClaudeCodeConfig {
    /// Instructions for Claude Code
    #[serde(default)]
    pub instruction: String,

    /// Working directory for Claude Code
    #[serde(default)]
    pub workdir: Option<PathBuf>,

    /// Docker image to use (if not using nix-shell)
    #[serde(rename = "docker-image")]
    pub docker_image: Option<String>,

    /// Docker image to build from nix-shell
    #[serde(rename = "docker-image-from")]
    pub docker_image_from: Option<String>,

    /// Path to nix-shell for building docker image
    #[serde(rename = "nix-shell")]
    pub nix_shell: Option<PathBuf>,

    /// Additional environment variables
    #[serde(default)]
    pub env: std::collections::HashMap<String, String>,

    /// Claude API key (optional, can use ~/.claude)
    #[serde(rename = "api-key")]
    pub api_key: Option<String>,

    /// Model to use
    #[serde(default)]
    pub model: Option<String>,
}

impl Default for ClaudeCodeConfig {
    fn default() -> Self {
        Self {
            instruction: String::new(),
            workdir: None,
            docker_image: None,
            docker_image_from: None,
            nix_shell: None,
            env: std::collections::HashMap::new(),
            api_key: None,
            model: None,
        }
    }
}

impl ClaudeCodeConfig {
    /// Parse configuration from TOML string
    pub fn parse(toml_str: &str) -> Result<Self> {
        toml::from_str(toml_str)
            .map_err(|e| anyhow!("Failed to parse config TOML: {}", e))
    }

    /// Get the effective working directory
    pub fn effective_workdir(&self) -> PathBuf {
        self.workdir.clone()
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/")))
    }

    /// Check if Docker should be used
    pub fn use_docker(&self) -> bool {
        self.docker_image.is_some() || self.docker_image_from.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_config() {
        let toml = r#"
            instruction = "the main code is in protocol-2026"
            workdir = "/home/user/project"
            docker-image-from = "nix-shell"
            nix-shell = "/home/user/project/shell.nix"
        "#;

        let config = ClaudeCodeConfig::parse(toml).unwrap();
        assert_eq!(config.instruction, "the main code is in protocol-2026");
        assert_eq!(config.workdir, Some(PathBuf::from("/home/user/project")));
        assert_eq!(config.docker_image_from, Some("nix-shell".to_string()));
    }

    #[test]
    fn test_parse_minimal_config() {
        let toml = r#"
            instruction = "help me with this code"
        "#;

        let config = ClaudeCodeConfig::parse(toml).unwrap();
        assert_eq!(config.instruction, "help me with this code");
        assert!(config.workdir.is_none());
    }
}
