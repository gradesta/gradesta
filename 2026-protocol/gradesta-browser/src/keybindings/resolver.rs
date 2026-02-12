//! Keybinding resolution - merges defaults with user config

use std::collections::HashMap;

use crate::commands::{Command, Context};
use crate::keybindings::config::KeybindingsConfig;
use crate::keybindings::defaults;
use crate::keybindings::key::KeyBinding;

/// Resolves keybindings from user config + defaults
pub struct KeybindingResolver {
    /// Map from (context, key_binding) -> command
    bindings: HashMap<(Context, KeyBinding), Command>,
    /// Reverse map: command -> list of bindings (for display)
    reverse: HashMap<Command, Vec<KeyBinding>>,
}

impl Default for KeybindingResolver {
    fn default() -> Self {
        Self::new(&KeybindingsConfig::default())
    }
}

impl KeybindingResolver {
    /// Create a new resolver with user config merged over defaults
    pub fn new(user_config: &KeybindingsConfig) -> Self {
        let mut resolver = Self::with_defaults();
        resolver.apply_user_config(user_config);
        resolver
    }

    /// Create a resolver with only default bindings
    fn with_defaults() -> Self {
        let mut bindings = HashMap::new();
        let mut reverse: HashMap<Command, Vec<KeyBinding>> = HashMap::new();

        for (command, context, binding) in defaults::all_defaults() {
            bindings.insert((context, binding.clone()), command.clone());
            reverse.entry(command).or_default().push(binding);
        }

        Self { bindings, reverse }
    }

    /// Apply user config overrides
    fn apply_user_config(&mut self, config: &KeybindingsConfig) {
        // Helper to apply bindings from a context section
        let apply_context = |resolver: &mut Self, context: Context, section: &HashMap<String, crate::keybindings::config::KeyBindingValue>| {
            for (action, value) in section {
                // Build the full slug
                let ctx_name = match context {
                    Context::Global => "global",
                    Context::Graph => "graph",
                    Context::Bag => "bag",
                    Context::TextInput => "text_input",
                    Context::Recording => "recording",
                    Context::Authentication => "auth",
                    Context::NavPanel => "nav_panel",
                    Context::Export => "export",
                };
                let slug = format!("{}.{}", ctx_name, action);

                if let Some(command) = Command::from_slug(&slug) {
                    // Remove old bindings for this command
                    resolver.remove_bindings_for(&command);

                    // Add new bindings
                    for key_str in value.as_strings() {
                        if let Ok(binding) = KeyBinding::parse(key_str) {
                            resolver.add_binding(context, binding, command.clone());
                        } else {
                            eprintln!("Warning: Invalid keybinding '{}' for {}", key_str, slug);
                        }
                    }
                }
            }
        };

        apply_context(self, Context::Global, &config.global);
        apply_context(self, Context::Graph, &config.graph);
        apply_context(self, Context::Bag, &config.bag);
        apply_context(self, Context::TextInput, &config.text_input);
        apply_context(self, Context::Recording, &config.recording);
        apply_context(self, Context::Authentication, &config.authentication);
        apply_context(self, Context::NavPanel, &config.nav_panel);
    }

    /// Remove all bindings for a command
    fn remove_bindings_for(&mut self, command: &Command) {
        // Remove from forward map
        self.bindings.retain(|_, cmd| cmd != command);
        // Clear reverse map entry
        self.reverse.remove(command);
    }

    /// Add a binding
    fn add_binding(&mut self, context: Context, binding: KeyBinding, command: Command) {
        self.bindings.insert((context, binding.clone()), command.clone());
        self.reverse.entry(command).or_default().push(binding);
    }

    /// Resolve a key press to a command
    /// Checks the specific context first, then falls back to Global
    pub fn resolve(&self, context: Context, binding: &KeyBinding) -> Option<Command> {
        // Check context-specific binding first
        if let Some(cmd) = self.bindings.get(&(context, binding.clone())) {
            return Some(cmd.clone());
        }
        // Fall back to global if not in global context already
        if context != Context::Global {
            if let Some(cmd) = self.bindings.get(&(Context::Global, binding.clone())) {
                return Some(cmd.clone());
            }
        }
        None
    }

    /// Get all bindings for a command
    pub fn get_bindings(&self, command: &Command) -> Vec<KeyBinding> {
        self.reverse.get(command).cloned().unwrap_or_default()
    }

    /// Get all bindings organized by context
    pub fn all_bindings_by_context(&self) -> HashMap<Context, Vec<(Command, Vec<KeyBinding>)>> {
        let mut result: HashMap<Context, Vec<(Command, Vec<KeyBinding>)>> = HashMap::new();

        for cmd in Command::all() {
            let ctx = cmd.context();
            let bindings = self.get_bindings(&cmd);
            result.entry(ctx).or_default().push((cmd, bindings));
        }

        result
    }

    /// Rebind a command to new key(s)
    /// Returns the old bindings
    pub fn rebind(&mut self, command: &Command, new_bindings: Vec<KeyBinding>) -> Vec<KeyBinding> {
        let old = self.get_bindings(command);
        let context = command.context();

        // Remove old bindings
        self.remove_bindings_for(command);

        // Add new bindings
        for binding in new_bindings {
            self.add_binding(context, binding, command.clone());
        }

        old
    }

    /// Reset a command to its default bindings
    pub fn reset_to_default(&mut self, command: &Command) {
        self.remove_bindings_for(command);

        let context = command.context();
        for binding in defaults::get_default_bindings(command) {
            self.add_binding(context, binding, command.clone());
        }
    }

    /// Reset all bindings to defaults
    pub fn reset_all_to_defaults(&mut self) {
        self.bindings.clear();
        self.reverse.clear();

        for (command, context, binding) in defaults::all_defaults() {
            self.bindings.insert((context, binding.clone()), command.clone());
            self.reverse.entry(command).or_default().push(binding);
        }
    }

    /// Check if a key binding conflicts with existing bindings in a context
    pub fn find_conflict(&self, context: Context, binding: &KeyBinding) -> Option<Command> {
        self.bindings.get(&(context, binding.clone())).cloned()
            .or_else(|| self.bindings.get(&(Context::Global, binding.clone())).cloned())
    }

    /// Check if a specific command was triggered by any key press in the current frame
    pub fn command_pressed(&self, context: Context, command: &Command, input: &bevy_egui::egui::InputState) -> bool {
        // Get all bindings for this command
        if let Some(bindings) = self.reverse.get(command) {
            for binding in bindings {
                if binding.is_pressed(input) {
                    // Verify the resolved command matches (handles context properly)
                    if self.resolve(context, binding).as_ref() == Some(command) {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Check if a specific command's key is currently held down
    pub fn command_down(&self, context: Context, command: &Command, input: &bevy_egui::egui::InputState) -> bool {
        if let Some(bindings) = self.reverse.get(command) {
            for binding in bindings {
                if binding.is_down(input) {
                    if self.resolve(context, binding).as_ref() == Some(command) {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Check if a specific command's key was released this frame
    pub fn command_released(&self, context: Context, command: &Command, input: &bevy_egui::egui::InputState) -> bool {
        if let Some(bindings) = self.reverse.get(command) {
            for binding in bindings {
                if binding.is_released(input) {
                    // Note: For releases, we check if this binding WOULD resolve to the command
                    // (modifiers are ignored on release, so we just check the key)
                    if self.resolve(context, binding).as_ref() == Some(command) {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Check if a command's key is pressed using Bevy input (for key repeat handling)
    /// Returns true if any binding for this command is currently pressed
    pub fn command_pressed_bevy(
        &self,
        context: Context,
        command: &Command,
        keys: &bevy::prelude::ButtonInput<bevy::prelude::KeyCode>,
    ) -> bool {
        if let Some(bindings) = self.reverse.get(command) {
            for binding in bindings {
                // Check modifiers match
                let mods = crate::keybindings::Modifiers::from_bevy(keys);
                if mods != binding.modifiers {
                    continue;
                }
                // Convert our key to bevy key and check if pressed
                if let Some(bevy_key) = binding.key.to_bevy() {
                    if keys.pressed(bevy_key) {
                        if self.resolve(context, binding).as_ref() == Some(command) {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }

    /// Check if a command's key was just pressed using Bevy input
    pub fn command_just_pressed_bevy(
        &self,
        context: Context,
        command: &Command,
        keys: &bevy::prelude::ButtonInput<bevy::prelude::KeyCode>,
    ) -> bool {
        if let Some(bindings) = self.reverse.get(command) {
            for binding in bindings {
                let mods = crate::keybindings::Modifiers::from_bevy(keys);
                if mods != binding.modifiers {
                    continue;
                }
                if let Some(bevy_key) = binding.key.to_bevy() {
                    if keys.just_pressed(bevy_key) {
                        if self.resolve(context, binding).as_ref() == Some(command) {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }

    /// Save current bindings to config file
    /// Only saves bindings that differ from defaults
    pub fn save_to_config(&self) -> anyhow::Result<()> {
        let mut config = KeybindingsConfig::default();

        // For each command, check if its bindings differ from defaults
        for command in Command::all() {
            let current_bindings = self.get_bindings(&command);
            let default_bindings = defaults::get_default_bindings(&command);

            // Skip if bindings match defaults
            if current_bindings == default_bindings {
                continue;
            }

            // Build the action name from the slug
            let slug = command.slug();
            let parts: Vec<&str> = slug.splitn(2, '.').collect();
            if parts.len() != 2 {
                continue;
            }
            let action = parts[1].to_string();

            // Convert bindings to strings
            let binding_strs: Vec<String> = current_bindings.iter()
                .map(|b| b.to_string())
                .collect();

            let value = if binding_strs.len() == 1 {
                crate::keybindings::config::KeyBindingValue::Single(binding_strs[0].clone())
            } else {
                crate::keybindings::config::KeyBindingValue::Multiple(binding_strs)
            };

            // Add to appropriate context section
            match command.context() {
                Context::Global => { config.global.insert(action, value); }
                Context::Graph => { config.graph.insert(action, value); }
                Context::Bag => { config.bag.insert(action, value); }
                Context::TextInput => { config.text_input.insert(action, value); }
                Context::Recording => { config.recording.insert(action, value); }
                Context::Authentication => { config.authentication.insert(action, value); }
                Context::NavPanel => { config.nav_panel.insert(action, value); }
                Context::Export => { config.export.insert(action, value); }
            }
        }

        config.save()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keybindings::key::KeyCode;

    #[test]
    fn test_default_resolution() {
        let resolver = KeybindingResolver::default();

        // Ctrl+L should resolve to GlobalFocusUrl
        let binding = KeyBinding::parse("Ctrl+L").unwrap();
        let cmd = resolver.resolve(Context::Graph, &binding);
        assert_eq!(cmd, Some(Command::GlobalFocusUrl));
    }

    #[test]
    fn test_context_specific() {
        let resolver = KeybindingResolver::default();

        // Y in Graph context should be yank
        let binding = KeyBinding::simple(KeyCode::Y);
        let cmd = resolver.resolve(Context::Graph, &binding);
        assert_eq!(cmd, Some(Command::GraphYank));
    }

    #[test]
    fn test_global_fallback() {
        let resolver = KeybindingResolver::default();

        // Escape should work in any context (global command)
        let binding = KeyBinding::simple(KeyCode::Escape);
        let cmd = resolver.resolve(Context::Graph, &binding);
        assert_eq!(cmd, Some(Command::GlobalCloseModal));
    }
}
