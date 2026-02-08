//! Keybinding presets - predefined keybinding configurations

use crate::keybindings::config::{KeyBindingValue, KeybindingsConfig};
use std::collections::HashMap;

/// Available keybinding presets
#[derive(Clone, Debug, PartialEq)]
pub enum Preset {
    /// Windows/standard keybindings (current defaults)
    Normal,
    /// Vim-style keybindings (hjkl navigation, etc.)
    VimLike,
    /// Emacs-style keybindings (Ctrl+N/P/F/B navigation, etc.)
    EmacsLike,
}

impl Preset {
    /// Get all available presets
    pub fn all() -> &'static [Preset] {
        &[Preset::Normal, Preset::VimLike, Preset::EmacsLike]
    }

    /// Get the display name of this preset
    pub fn name(&self) -> &'static str {
        match self {
            Preset::Normal => "Normal",
            Preset::VimLike => "Vim",
            Preset::EmacsLike => "Emacs",
        }
    }

    /// Get a description of this preset
    pub fn description(&self) -> &'static str {
        match self {
            Preset::Normal => "Standard keybindings (arrows, Ctrl+shortcuts)",
            Preset::VimLike => "Vim-style (hjkl navigation, modal commands)",
            Preset::EmacsLike => "Emacs-style (Ctrl+N/P/F/B navigation)",
        }
    }

    /// Get the keybinding overrides for this preset
    /// These are merged into the existing config (not a complete replacement)
    pub fn bindings(&self) -> KeybindingsConfig {
        match self {
            Preset::Normal => normal_bindings(),
            Preset::VimLike => vim_bindings(),
            Preset::EmacsLike => emacs_bindings(),
        }
    }
}

/// Merge preset bindings into existing config
/// This is a dictionary update - preset values override existing ones
pub fn apply_preset(existing: &mut KeybindingsConfig, preset: &Preset) {
    let preset_config = preset.bindings();

    merge_section(&mut existing.global, &preset_config.global);
    merge_section(&mut existing.graph, &preset_config.graph);
    merge_section(&mut existing.bag, &preset_config.bag);
    merge_section(&mut existing.text_input, &preset_config.text_input);
    merge_section(&mut existing.recording, &preset_config.recording);
    merge_section(&mut existing.authentication, &preset_config.authentication);
    merge_section(&mut existing.nav_panel, &preset_config.nav_panel);
}

/// Merge one section's bindings into another
fn merge_section(
    target: &mut HashMap<String, KeyBindingValue>,
    source: &HashMap<String, KeyBindingValue>,
) {
    for (key, value) in source {
        target.insert(key.clone(), value.clone());
    }
}

/// Helper to create a KeyBindingValue from strings
fn keys(bindings: &[&str]) -> KeyBindingValue {
    if bindings.len() == 1 {
        KeyBindingValue::Single(bindings[0].to_string())
    } else {
        KeyBindingValue::Multiple(bindings.iter().map(|s| s.to_string()).collect())
    }
}

/// Normal preset - essentially empty (uses defaults)
fn normal_bindings() -> KeybindingsConfig {
    // Normal preset doesn't override anything - uses built-in defaults
    KeybindingsConfig::default()
}

/// Vim-like keybindings
fn vim_bindings() -> KeybindingsConfig {
    let mut config = KeybindingsConfig::default();

    // Graph context - navigation with hjkl
    config.graph.insert("navigate_west".to_string(), keys(&["Left", "A", "H"]));
    config.graph.insert("navigate_east".to_string(), keys(&["Right", "D", "L"]));
    config.graph.insert("navigate_north".to_string(), keys(&["Up", "W", "K"]));
    config.graph.insert("navigate_south".to_string(), keys(&["Down", "S", "J"]));

    // Page up/down with Ctrl+U/D
    config.graph.insert("navigate_up".to_string(), keys(&["PageUp", "Ctrl+U"]));
    config.graph.insert("navigate_down".to_string(), keys(&["PageDown", "Ctrl+D"]));

    // u for history back (undo-like)
    config.graph.insert("history_back".to_string(), keys(&["Backspace", "U"]));

    // o for new vertex (like vim's 'o' for new line)
    config.graph.insert("new_text_vertex".to_string(), keys(&["N", "O"]));

    // x for delete (like vim's 'x' for delete char)
    config.graph.insert("delete_vertex".to_string(), keys(&["Delete", "X"]));

    config
}

/// Emacs-like keybindings
fn emacs_bindings() -> KeybindingsConfig {
    let mut config = KeybindingsConfig::default();

    // Global context
    // Ctrl+G for cancel (universal emacs cancel)
    config.global.insert("close_modal".to_string(), keys(&["Escape", "Ctrl+G"]));

    // Move keybindings editor to Ctrl+Shift+K (Ctrl+K is taken for cut)
    config.global.insert("open_keybindings".to_string(), keys(&["Ctrl+Shift+K"]));

    // Graph context - Ctrl+F/B/N/P navigation
    config.graph.insert("navigate_west".to_string(), keys(&["Left", "A", "Ctrl+B"]));
    config.graph.insert("navigate_east".to_string(), keys(&["Right", "D", "Ctrl+F"]));
    config.graph.insert("navigate_north".to_string(), keys(&["Up", "W", "Ctrl+P"]));
    config.graph.insert("navigate_south".to_string(), keys(&["Down", "S", "Ctrl+N"]));

    // Page navigation with Ctrl+V / Alt+V
    config.graph.insert("navigate_up".to_string(), keys(&["PageUp", "Alt+V"]));
    config.graph.insert("navigate_down".to_string(), keys(&["PageDown", "Ctrl+V"]));

    // Ctrl+O for new vertex (like emacs open-line)
    config.graph.insert("new_text_vertex".to_string(), keys(&["N", "Ctrl+O"]));

    // Ctrl+D for delete
    config.graph.insert("delete_vertex".to_string(), keys(&["Delete", "Ctrl+D"]));

    // Ctrl+K for cut edge (like emacs kill-line)
    config.graph.insert("cut_edge".to_string(), keys(&["C", "Ctrl+K"]));

    // Alt+W for yank (copy), Ctrl+Y for paste
    config.graph.insert("yank".to_string(), keys(&["Y", "Alt+W"]));
    config.graph.insert("paste".to_string(), keys(&["P", "Ctrl+Y"]));

    config
}
