//! Keybindings configuration file handling

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// A keybinding value in the config - either a single key or multiple keys
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum KeyBindingValue {
    /// Single key binding (e.g., "Ctrl+N")
    Single(String),
    /// Multiple key bindings (e.g., ["ArrowUp", "W"])
    Multiple(Vec<String>),
}

impl KeyBindingValue {
    /// Get all key binding strings
    pub fn as_strings(&self) -> Vec<&str> {
        match self {
            KeyBindingValue::Single(s) => vec![s.as_str()],
            KeyBindingValue::Multiple(v) => v.iter().map(|s| s.as_str()).collect(),
        }
    }
}

/// User keybindings configuration
/// This is sparse - only contains user overrides
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct KeybindingsConfig {
    #[serde(default)]
    pub global: HashMap<String, KeyBindingValue>,
    #[serde(default)]
    pub graph: HashMap<String, KeyBindingValue>,
    #[serde(default)]
    pub bag: HashMap<String, KeyBindingValue>,
    #[serde(default)]
    pub text_input: HashMap<String, KeyBindingValue>,
    #[serde(default)]
    pub recording: HashMap<String, KeyBindingValue>,
    #[serde(default)]
    pub authentication: HashMap<String, KeyBindingValue>,
    #[serde(default)]
    pub nav_panel: HashMap<String, KeyBindingValue>,
    #[serde(default)]
    pub export: HashMap<String, KeyBindingValue>,
    #[serde(default)]
    pub elf: HashMap<String, KeyBindingValue>,
}

impl KeybindingsConfig {
    /// Get the config file path
    pub fn config_path() -> Result<PathBuf> {
        let config_dir = dirs::config_dir()
            .ok_or_else(|| anyhow!("Could not determine config directory"))?;
        Ok(config_dir.join("gradesta").join("keybindings.toml"))
    }

    /// Load keybindings config from disk
    /// Returns default (empty) config if file doesn't exist
    pub fn load() -> Result<Self> {
        let path = Self::config_path()?;
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(&path)?;
        let config: Self = toml::from_str(&content)?;
        Ok(config)
    }

    /// Save keybindings config to disk
    pub fn save(&self) -> Result<()> {
        let path = Self::config_path()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)?;
        std::fs::write(&path, content)?;
        Ok(())
    }
}
