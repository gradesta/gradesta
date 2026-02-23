//! Keybinding system
//!
//! Provides configurable keybindings with:
//! - TOML config file support
//! - Default bindings built-in
//! - Context-aware resolution (global fallback)
//! - Presets (Normal, Vim, Emacs)

#![allow(dead_code)]

pub mod config;
pub mod defaults;
pub mod key;
pub mod presets;
pub mod resolver;

pub use config::KeybindingsConfig;
pub use key::{KeyBinding, Modifiers};
pub use presets::Preset;
pub use resolver::KeybindingResolver;
